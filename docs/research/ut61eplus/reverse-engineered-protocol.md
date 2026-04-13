# UT61E+ Protocol: Reverse-Engineered Specification

Based on:
- UT61E+ User Manual (UNI-T)
- CP2110 Datasheet (Silicon Labs)
- AN434: CP2110/4 Interface Specification (Silicon Labs)
- UNI-T UT61E+ Software V2.02 (decompiled with Ghidra)

Confidence levels:
- **[KNOWN]** — established facts from official Silicon Labs documentation
- **[VENDOR]** — confirmed by decompiling UNI-T's official Windows software
- **[DEDUCED]** — logical inferences not yet verified against real hardware
- **[UNVERIFIED]** — requires real device testing to confirm
- **[VERIFIED]** — confirmed against a real UT61E+ device

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

### 1.4 Initialization Sequence — [KNOWN + VENDOR]

The CP2110.dll dynamically loads `SLABHIDtoUART.dll` and resolves these
function pointers:

1. `HidUart_Open` — open device by VID/PID
2. `HidUart_SetUartEnable(1)` — enable UART
3. `HidUart_SetUartConfig(9600, 0, 0, 3, 0)` — 9600/8N1/no flow control
4. `HidUart_FlushBuffers` — clear FIFOs
5. `HidUart_SetTimeouts(100, 100)` — 100ms read/write timeout

Our own implementation uses three raw HID feature reports (the same
underlying operations, without the SLABHIDtoUART wrapper):

1. **Enable UART:** `[0x41, 0x01]`
2. **Configure 9600/8N1:** `[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x00, 0x03, 0x00]`
   - Bytes 1-4: baud rate = `0x00002580` = 9600 (big-endian)
   - Byte 5: `0x00` = no parity
   - Byte 6: `0x00` = no flow control
   - Byte 7: `0x03` = 8 data bits
   - Byte 8: `0x00` = short stop bit (1 stop bit)
3. **Purge RX FIFO:** `[0x43, 0x02]` (0x01=TX, 0x02=RX, 0x03=both — RX only
   since TX is empty at init)

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

**Set Reset Device (report 0x40)** — Set (host → device), payload `[0x40, 0x00]`.
Resets the CP2110 and re-enumerates on USB. All UART config is lost — must
re-initialize after re-opening.

**[VERIFIED] UT61E+ quirk:** Report 0x40 is rejected with a HID protocol
error on the UT61E+'s CP2110. UNI-T likely locked this report out in the
device's HID descriptor.

### 1.6 CH9329 Alternate Transport — [VENDOR + DEDUCED]

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

**[UNVERIFIED]** CH9329 support has not been exercised against real
hardware. The HID framing above is deduced from the CH9329 datasheet and
the vendor `CH9329DLL.dll` filename; byte-for-byte UART compatibility
with the CP2110 path is assumed but unconfirmed.

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

### 2.2 Request/Response Model — [VENDOR]

The software uses a **polled** model. From `FUN_100016d0` (MyDmm
constructor):

- A `LoopCommandPool` continuously sends **GetMeasurement** commands
  (command byte 0x5E) on a timer
- An `OnceCommandPool` sends one-shot commands (Hold, Range, etc.)
- Default polling interval: 1000 ms (can be configured in `options.xml`
  via `SampleRate`)

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
| 0x46 | `F` | Range | `QByteArray::append('F')` in FUN_100021f0 |

**Probable commands** (from DMM.exe UI action names, not yet seen in
decompiled code — DMM.exe decompilation was incomplete):

| Probable Byte | Name | UI Action | Hardware Status |
|--------------|------|-----------|-----------------|
| 0x41 | MinMax toggle | `actionMaxMin` | **[VERIFIED]** (remote) |
| 0x42 | ExitMinMax | `actionExitMaxMin` | **[VERIFIED]** (remote) |
| 0x47 | Auto | `actionRangeAuto` | **[VERIFIED]** (restores auto-range) |
| 0x48 | Rel | `actionRel` | **[VERIFIED]** (remote) |
| 0x49 | Select2 (Hz/USB) | `actionHz` | **[VERIFIED]** (AC mV: cycles mV → Hz → Duty% → mV; no effect on DC V) |
| 0x4B | Light | `actionLight` | **[VERIFIED]** (backlight toggle) |
| 0x4C | Select (orange) | `actionSelect` | **[VERIFIED]** (cycles sub-modes, e.g. DC V → AC+DC V) |
| 0x4D | PeakMinMax | `actionPeak` | **[VERIFIED]** (AC modes only; beeps but no visible effect on DC V) |
| 0x4E | ExitPeak | `actionExitPeak` | **[VERIFIED]** (clears peak flags, returns to live) |
| 0x5F | GetName | (device discovery) | **[UNVERIFIED]** |

Hardware verification: commands issued against a real UT61E+ via `dmm-cli`
command tools; effects observed on the meter LCD and subsequent response
frames.

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
| 16 | 1 | Flags3 | bar_pol, P-MIN, P-MAX, DC |
| 17-18 | 2 | Checksum | 16-bit BE sum of bytes 0-16 |

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
| 0x03 | DC Millivolt | DCmV | String table position | [UNVERIFIED] |
| 0x04 | Frequency | FREQ | Multiplier check `cVar1 == '\x04'` | **[VERIFIED]** (V~ and mA via SELECT2) |
| 0x05 | Duty Cycle | Duty Cycle | Bar graph "-" check `cVar1 == '\x05'` | **[VERIFIED]** (mA via SELECT2) |
| 0x06 | Resistance | RES | Multiplier check `cVar1 == '\x06'` | **[VERIFIED]** |
| 0x07 | Continuity | Short-Circuit | Multiplier check `cVar1 == '\a'` (0x07) | **[VERIFIED]** |
| 0x08 | Diode | Diode | String table position | **[VERIFIED]** |
| 0x09 | Capacitance | CAP | Multiplier check `cVar1 == '\t'` (0x09) | **[VERIFIED]** |
| 0x0A | Temperature °C | Celsius | Special handling `cVar1 == '\n'` (0x0A) | — (not on UT61E+) |
| 0x0B | Temperature °F | Fahrenheit | Special handling `cVar1 == '\v'` (0x0B) | — (not on UT61E+) |
| 0x0C | DC µA | DCuA | String table position | **[VERIFIED]** |
| 0x0D | AC µA | ACuA | String table position | [UNVERIFIED] |
| 0x0E | DC mA | DCmA | String table position | **[VERIFIED]** |
| 0x0F | AC mA | ACmA | String table position | [UNVERIFIED] |
| 0x10 | DC A | DCA | String table position | **[VERIFIED]** (A⎓ dial) |
| 0x11 | AC A | ACA | String table position | **[VERIFIED]** (A⎓ + SELECT) |
| 0x12 | hFE | hFE | Multiplier check `cVar1 == '\x12'` | **[VERIFIED]** |
| 0x13 | Live | Live | Bar graph "-" check `cVar1 == '\x13'` | [UNVERIFIED] |
| 0x14 | NCV | NCV | Multiplier/"-" checks `cVar1 == '\x14'` | **[VERIFIED]** |
| 0x15 | LoZ Voltage | LozV | String table position | — (not on UT61E+) |
| 0x16 | LoZ Voltage 2 | LozV | Multiplier check `cVar1 == '\x16'` | — (not on UT61E+) |
| 0x17 | LPF | LPF | Bar graph "-" check `cVar1 == '\x17'` | — (not on UT61E+) |
| 0x18 | LPF V | | Gap in string table | **[VERIFIED]** (V~ + SELECT; no signal needed) |
| 0x19 | AC+DC V | AC+DC | AC/DC flag check `cVar1 == '\x19'` | **[VERIFIED]** (V⎓ + SELECT; no signal needed) |

Hardware-verified entries come from driving the real UT61E+ through each
physical dial position and observing the mode byte in the response frame.
The mode byte reflects the *active* measurement unit, not the dial
position — e.g. on DC V dial with auto-range, the meter reports 0x02 (DCV)
even when showing mV-scale values. The range byte determines the actual
scale.

**Speculative mode bytes 0x1A-0x1E:** extrapolated from the 0x18/0x19
pattern in earlier protocol notes (LPF and AC+DC variants on mV and A
ranges, plus Inrush). These do not appear in the vendor software's mode
string table (which ends at 0x19) and have not been observed from the
UT61E+. They are kept here for cross-referencing against other family
members that may use a superset of the UT61E+ mode table.

| Byte | Mode | Hardware Status |
|------|------|-----------------|
| 0x1A | LPF mV | [UNVERIFIED] |
| 0x1B | AC+DC mV | [UNVERIFIED] |
| 0x1C | LPF A | [UNVERIFIED] |
| 0x1D | AC+DC A | [UNVERIFIED] |
| 0x1E | Inrush | [UNVERIFIED] |

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
equivalently `range_byte - 0x30`).

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

where `byte12` is the tens digit and `byte13` is the ones digit.
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

**Byte 14 (offset 0x0E) — Flags1:**

| Bit | Mask | Flag | Status | Evidence |
|-----|------|------|--------|----------|
| 0 | 0x01 | REL | **[VERIFIED]** | `bVar3 & 1` → `FUN_10008830(this, ...)` |
| 1 | 0x02 | HOLD | **[VERIFIED]** | `bVar3 >> 1 & 1` → `FUN_10008ca0(this, ...)` |
| 2 | 0x04 | MIN | **[VERIFIED]** | `(bVar3 & 4) → "MIN"` |
| 3 | 0x08 | MAX | **[VERIFIED]** | `(bVar3 & 8) → "MAX"` |

**[VERIFIED] MIN/MAX cycle:** MAX only → MIN only → MAX (2-state, bits
never both set). When MIN or MAX is set, the `display` field contains
the *stored* extremum, not the live reading. The AUTO flag is cleared
(range locked) for the duration of MIN/MAX mode.

**Byte 15 (offset 0x0F) — Flags2:**

| Bit | Mask | Flag | Status | Evidence |
|-----|------|------|--------|----------|
| 0 | 0x01 | **HV warning** | **[VERIFIED]** | Set at 31V on DC V (manual: >30V). DmmData offset 0x3d — stored but not displayed by PC UI. |
| 1 | 0x02 | **Low battery** | **[VERIFIED]** | Intermittent on real device. DmmData offset 0x3c — passed to a UI indicator widget. |
| 2 | 0x04 | **!AUTO** (inverted) | **[VERIFIED]** | DmmData offset 0x3b. Bit CLEAR = auto-range ON. DMM.exe hides "AUTO" label when set. |
| 3 | 0x08 | (reserved) | [UNVERIFIED] | Stored at DmmData offset 0x3a; never read by PC UI. |

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
| 0 | 0x01 | bar_pol | **[VERIFIED]** | Set when the reading is negative; bar graph then holds the magnitude. |
| 1 | 0x02 | P-MIN | **[VERIFIED]** | `(bVar3 & 2) → "P-MIN"` |
| 2 | 0x04 | P-MAX | **[VERIFIED]** | `(bVar3 & 4) → "P-MAX"` |
| 3 | 0x08 | DC indicator | [VENDOR] | `(bVar3 & 8)` → used in AC+DC mode for AC/DC distinction |

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
  bytes one at a time via HID interrupt reports. Accumulate in a buffer
  and scan for complete `AB CD` frames. A full measurement response
  requires ~19 individual reads.
- **Timeout vs disconnect:** `HidDevice::read_timeout()` returns 0 on
  timeout and an error on USB disconnect — handle both cases.
- **Request-response only:** the meter never streams data; each reading
  requires sending the `0x5E` request command.
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

### Resolved by deeper decompilation analysis

- **AUTO flag**: CONFIRMED inverted at byte 15 bit 2 (bit set = manual,
  bit clear = auto). The DMM.exe UI code explicitly hides the "AUTO"
  label when this bit is set.
- **Mode byte masking**: NOT NEEDED. Mode byte is used raw (0x00-0x19)
  in both CustomDmm.dll and DMM.exe — no `& 0x0F` anywhere.
- **Range byte masking**: The vendor software applies NO masking to the
  range byte. It's passed directly to the table lookup.
- **Flag byte 15 partial**: bit2 = !AUTO (confirmed). bit1 = passed to a
  non-text UI widget (later verified as Low Battery indicator). bits 0
  and 3 are stored but never displayed by the PC software.

### Resolved by hardware verification against real UT61E+

- **Range byte 0x30 prefix**: CONFIRMED — the meter does send 0x30-prefixed
  range bytes; mask with `& 0x0F` (or subtract 0x30) to get the index.
- **Flag byte 14 bits 0-3**: REL, HOLD, MIN, MAX — all confirmed, plus
  MAX → MIN → MAX 2-state cycle and stored-value display semantics.
- **Flag byte 15 bit 0**: HV warning — set at 31V on DC V (manual: >30V).
- **Flag byte 15 bit 1**: Low Battery — confirmed (intermittent).
- **Flag byte 16 bits 0-2**: bar_pol, P-MIN, P-MAX — all confirmed, plus
  P-MAX → P-MIN → P-MAX 2-state cycle and stored-value display semantics.
- **Bar graph encoding**: `byte12 * 10 + byte13` (raw decimal digits,
  no 0x30 prefix). Negative values store magnitude in the bar graph and
  set bar_pol; OL reads 44 segments.
- **Additional commands**: 0x41/0x42/0x47/0x48/0x49/0x4B/0x4C/0x4D/0x4E
  all exercised via the CLI and observed to produce the expected effect
  on the meter. 0x49 (Hz/Duty) and 0x4D (Peak) are silently context-
  dependent — they beep but produce no visible effect on DC V.
- **Timing**: ~100 ms round-trip per request/response at 9600 baud.
  Maximum sustained rate ~10 Hz. 19200/115200 baud both tested — meter
  does not respond.
- **Mode table (most entries)**: see §2.5 for per-mode verification status.

### Must Verify Against Real Hardware

1. **Flag byte 15 bit 3**: reserved/unused? Stored by PC software but
   never read; no observed behavior yet.
2. **Commands 0x5F (GetName)**: not yet issued against hardware.
3. **Mode bytes 0x03, 0x0D, 0x0F, 0x13**: not exercised (DC mV, AC µA,
   AC mA, Live). 0x0A/0x0B (temperature) are UT61D+ only.
4. **Speculative mode bytes 0x1A-0x1E**: not yet observed from any device.
5. **Edge cases**: NCV display format, hFE display format, temperature
   handling on UT61D+, OL in different modes.
6. **CH9329 transport**: has not been exercised against a real UT61E+.

---

## 6. Summary of Confidence Levels

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
| Flags3 at byte[16]: P-MIN/P-MAX/DC | **VENDOR** | `FUN_10007d50` bit operations |
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
| Byte15 bit2 = !AUTO (inverted) | **VENDOR** | DMM.exe UI hides "AUTO" when set |
| Byte15 bit1 = UI indicator widget | **VENDOR** | DMM.exe passes to widget method |
| Byte15 bits 0,3 = stored, not displayed | **VENDOR** | DMM.exe never reads back |
| Bar graph bytes (12-13) not used by vendor | **VENDOR** | No reads in display function |
| Byte15 bit0 = HV warning | **VERIFIED** | Set at 31V on DC V (real device) |
| Byte15 bit1 = Low Battery | **VERIFIED** | Observed intermittently on real device |
| Byte15 bit3 flag name | **UNVERIFIED** | Stored by PC software but never read |
| Bar graph position encoding (bytes 12-13) | **VERIFIED** | `byte12*10 + byte13` decimal, real device |
| Commands 0x41/0x42/0x47/0x48/0x49/0x4B/0x4C/0x4D/0x4E | **VERIFIED** | Exercised against real UT61E+ via CLI |
| Command 0x5F (GetName) | **UNVERIFIED** | Not in vendor software V2.02; not yet issued to real device |
| Sampling rate ~10 Hz at 9600 baud | **VERIFIED** | Measured throughput, 19200/115200 unresponsive |
| MIN/MAX and Peak 2-state cycles | **VERIFIED** | MAX → MIN → MAX, P-MAX → P-MIN → P-MAX |
| Range byte 0x30 prefix sent by meter | **VERIFIED** | Real device observation |
| CH9329 alternate transport | **UNVERIFIED** | Framing deduced from datasheet, no device tested |
