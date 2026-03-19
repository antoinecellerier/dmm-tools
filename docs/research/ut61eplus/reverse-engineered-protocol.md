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

| Probable Byte | Name | UI Action |
|--------------|------|-----------|
| 0x41 | MinMax | `actionMaxMin` |
| 0x42 | ExitMinMax | `actionExitMaxMin` |
| 0x47 | Auto | `actionRangeAuto` |
| 0x48 | Rel | `actionRel` |
| 0x49 | Select2 | `actionHz` |
| 0x4B | Light | `actionLight` |
| 0x4C | Select | `actionSelect` |
| 0x4D | PeakMinMax | `actionPeak` |
| 0x4E | ExitPeak | `actionExitPeak` |
| 0x5F | GetName | (device discovery) |

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

**Overload detection** (from `FUN_100026a0`):
- If display contains "O" AND "L" → OL (overload)
- If display also contains "-" → negative OL
- Returns: 0 = normal, 1 = negative OL, 2 = positive OL

### 2.5 Mode Byte Values — [VENDOR]

From the mode string lookup table at 0xD324 in CustomDmm.dll, and
confirmed by the mode-specific code paths in FUN_10007d50:

| Byte | Mode | Display Name | Confirmed By |
|------|------|-------------|--------------|
| 0x00 | AC Voltage | ACV | String table position |
| 0x01 | AC Millivolt | ACmV | String table position |
| 0x02 | DC Voltage | DCV | String table position |
| 0x03 | DC Millivolt | DCmV | String table position |
| 0x04 | Frequency | FREQ | Multiplier check `cVar1 == '\x04'` |
| 0x05 | Duty Cycle | Duty Cycle | Bar graph "-" check `cVar1 == '\x05'` |
| 0x06 | Resistance | RES | Multiplier check `cVar1 == '\x06'` |
| 0x07 | Continuity | Short-Circuit | Multiplier check `cVar1 == '\a'` (0x07) |
| 0x08 | Diode | Diode | String table position |
| 0x09 | Capacitance | CAP | Multiplier check `cVar1 == '\t'` (0x09) |
| 0x0A | Temperature °C | Celsius | Special handling `cVar1 == '\n'` (0x0A) |
| 0x0B | Temperature °F | Fahrenheit | Special handling `cVar1 == '\v'` (0x0B) |
| 0x0C | DC µA | DCuA | String table position |
| 0x0D | AC µA | ACuA | String table position |
| 0x0E | DC mA | DCmA | String table position |
| 0x0F | AC mA | ACmA | String table position |
| 0x10 | DC A | DCA | String table position |
| 0x11 | AC A | ACA | String table position |
| 0x12 | hFE | hFE | Multiplier check `cVar1 == '\x12'` |
| 0x13 | Live | Live | Bar graph "-" check `cVar1 == '\x13'` |
| 0x14 | NCV | NCV | Multiplier/"-" checks `cVar1 == '\x14'` |
| 0x15 | LoZ Voltage | LozV | String table position |
| 0x16 | LoZ Voltage 2 | LozV | Multiplier check `cVar1 == '\x16'` |
| 0x17 | LPF | LPF | Bar graph "-" check `cVar1 == '\x17'` |
| 0x18 | (unknown) | | Gap in string table |
| 0x19 | AC+DC | AC+DC | AC/DC flag check `cVar1 == '\x19'` |

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
The actual bar graph position data from the response bytes may be unused
by the PC software.

### 2.7 Flag Bytes — [VENDOR]

From FUN_10007d50 (response parser), the three flag bytes at offsets
14-16 have these bit assignments:

**Byte 14 (offset 0x0E) — Flags1:**

| Bit | Mask | Flag | Evidence |
|-----|------|------|----------|
| 0 | 0x01 | REL | `bVar3 & 1` → `FUN_10008830(this, ...)` |
| 1 | 0x02 | HOLD | `bVar3 >> 1 & 1` → `FUN_10008ca0(this, ...)` |
| 2 | 0x04 | MIN | `(bVar3 & 4) → "MIN"` |
| 3 | 0x08 | MAX | `(bVar3 & 8) → "MAX"` |

**Byte 15 (offset 0x0F) — Flags2:**

| Bit | Mask | Flag | Evidence |
|-----|------|------|----------|
| 0 | 0x01 | (stored, not displayed) | `bVar3 & 1` → DmmData offset 0x3d; never read by UI |
| 1 | 0x02 | (indicator widget) | `bVar3 >> 1 & 1` → DmmData offset 0x3c; passed to a UI widget method |
| 2 | 0x04 | **!AUTO** (inverted) | `bVar3 >> 2 & 1` → DmmData offset 0x3b; UI: when set, hides "AUTO" label |
| 3 | 0x08 | (stored, not displayed) | `bVar3 >> 3 & 1` → DmmData offset 0x3a; never read by UI |

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

**[DEDUCED]** Based on typical meter flag conventions: bit0 is likely HV
(high voltage alarm), bit1 is likely LowBat (passed to a visual
indicator widget, not a text label). Bits 0 and 3 are stored in the
DmmData object but never read back by the DMM.exe UI — they may only
be relevant for the meter's own LCD display.

**Byte 16 (offset 0x10) — Flags3:**

| Bit | Mask | Flag | Evidence |
|-----|------|------|----------|
| 0 | 0x01 | bar_pol | Likely (first flag read from byte 16) |
| 1 | 0x02 | P-MIN | `(bVar3 & 2) → "P-MIN"` |
| 2 | 0x04 | P-MAX | `(bVar3 & 4) → "P-MAX"` |
| 3 | 0x08 | DC | `(bVar3 & 8)` → used in AC+DC mode for AC/DC distinction |

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

## 5. What Still Requires Device Verification

### Resolved by deeper decompilation analysis

- **AUTO flag**: CONFIRMED inverted at byte 15 bit 2 (bit set = manual,
  bit clear = auto). The DMM.exe UI code explicitly hides the "AUTO"
  label when this bit is set.
- **Mode byte masking**: NOT NEEDED. Mode byte is used raw (0x00-0x19)
  in both CustomDmm.dll and DMM.exe — no `& 0x0F` anywhere.
- **Range byte masking**: The vendor software applies NO masking to the
  range byte. It's passed directly to the table lookup. Whether the meter
  sends raw or 0x30-prefixed values is still unknown, but the absence of
  masking code strongly suggests raw values.
- **Flag byte 15 partial**: bit2 = !AUTO (confirmed). bit1 = passed to a
  non-text UI widget (likely LowBat indicator). bits 0 and 3 are stored
  but never displayed by the PC software.

### Must Verify Against Real Hardware

1. **Flag byte 15 bit 0 and bit 3**: Likely HV and possibly reserved/unused.
   These are stored by the software but never displayed.

2. **Bar graph bytes (12-13)**: Not used by the vendor software. The
   bar graph full-scale range comes from the mode/range table, but the
   actual position encoding in the response is unknown.

3. **Additional commands**: Only 3 of ~13 expected commands confirmed
   (0x5E, 0x4A, 0x46). Searched ALL four decompiled binaries — no other
   command bytes are ever constructed. The vendor software V2.02 only
   implements GetMeasurement, Hold, and Range.

4. **Timing**: Actual response latency, maximum sustainable polling rate.

5. **Edge cases**: NCV display format, hFE display format, temperature
   handling, OL in different modes.

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
| Byte15 bit0/bit3 flag names | **UNVERIFIED** | Likely HV/reserved |
| Bar graph position encoding (bytes 12-13) | **UNVERIFIED** | Not used by vendor software |
| Commands beyond 0x5E/0x4A/0x46 | **UNVERIFIED** | Not in vendor software V2.02 |
