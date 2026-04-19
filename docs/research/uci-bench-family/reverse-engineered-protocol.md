# UCI Bench DMM Family: Reverse-Engineered Protocol Specification

Extension of the [UT8803 protocol specification](../ut8803/reverse-engineered-protocol.md)
to cover the remaining UCI bench DMM models.

Based on:
- UT8803E Programming Manual V1.1 (UNI-T)
- UNI-T SDK V2.3 (uci.dll)
- Ghidra decompilation of uci.dll (451K lines)

Confidence levels:
- **[KNOWN]** -- documented in official UNI-T programming manual
- **[VENDOR]** -- confirmed by analyzing UNI-T's official SDK/software
- **[DEDUCED]** -- logical inferences from available evidence
- **[UNVERIFIED]** -- requires real device testing to confirm
- **[MANUAL]** -- from per-model user manual or datasheet

---

## 1. UCI Protocol Layer (Shared)

All models in this family share the UCI (United Communication Interface)
protocol layer. The shared protocol details -- DMFRM struct, `data?;`
and `disp?;` commands, 64-bit flags word, functional/position coding,
and unit tables -- are fully documented in
[docs/research/ut8803/reverse-engineered-protocol.md](../ut8803/reverse-engineered-protocol.md).

This document covers only what differs from or extends the UT8803 spec:
- Transport variants (CP2110 vs QinHeng HID vs serial)
- UT8802 wire protocol (0xAC 8-byte BCD frames)
- QinHeng HID initialization
- Serial transport (UT805A)
- Per-model coding and range tables

---

## 2. Transport Variants

### 2.1 Overview -- [KNOWN]

The UCI bench DMMs use three different USB-to-serial transports:

| Transport | USB VID:PID | Models | Init in uci.dll |
|-----------|-------------|--------|-----------------|
| CP2110 (Silicon Labs) | 0x10C4:0xEA80 | UT8802, UT8803 | FUN_1001d460 |
| QinHeng HID (CH9325) | 0x1A86:0xE008 | UT632, UT803, UT804 | FUN_1001d360 / FUN_1001d270 |
| Serial (USB-to-COM) | N/A (COM port) | UT805A | CSerialPort class |

### 2.2 Connection Dispatch -- [VENDOR]

From `FUN_1001ef50` (line 25322) in uci.dll:

```
1. Read USB VID/PID from device info struct
2. If VID=0x10C4, PID=0xEA80 (CP2110):
   a. Run CP2110 init (FUN_1001d460)
   b. Sleep 500ms
   c. Set device_type = 4 (UT8802)
   d. Probe: try 2 reads within 300ms
   e. If probe fails:
      - Re-init CP2110
      - Set device_type = 5 (UT8803)
      - Probe with 2000ms timeout
   f. If still fails: abort
3. If VID=0x1A86, PID=0xE008 (QinHeng):
   a. Run primary QinHeng init (FUN_1001d360, with 0x5A trigger)
   b. Sleep 500ms
   c. Set device_type = 0 (QinHeng primary)
   d. Probe: try 2 reads within 300ms
   e. If probe fails:
      - Run fallback QinHeng init (FUN_1001d270, no trigger)
      - Sleep 500ms
      - Set device_type = 1 (QinHeng fallback)
      - Probe with 300ms timeout
   f. If still fails: abort
4. Any other VID/PID: reject
```

**Device type values** (stored at offset 0x10 from connection context):

| Value | Meaning | Wire Format |
|-------|---------|-------------|
| 0 | QinHeng primary (with trigger) | Auto-detect |
| 1 | QinHeng fallback (no trigger) | Auto-detect |
| 4 | CP2110, UT8802 | 0xAC 8-byte |
| 5 | CP2110, UT8803 | 0xABCD 21-byte |

### 2.3 Frame Auto-Detection -- [VENDOR]

When the device type is QinHeng (0 or 1), or during initial probing,
the frame discriminator (`FUN_1001eb30`, line 25103) auto-detects the
wire format by examining frame headers:

```
Scan incoming HID data for header bytes:
  - If first byte == 0xAC:
      → UT8802 format (8-byte frame)
      → Log: "[UT880X] Parse is UT8802"
  - If first byte == 0xAB AND second byte == 0xCD:
      → UT8803 format (21-byte frame)
      → Log: "[UT880X] Parse is UT8803"
  - Otherwise: unknown (-1)
```

This auto-detect logic applies to the UCI SDK (uci.dll) only. **The
actual UT803 and UT804 meters do NOT use 0xAC or 0xABCD format.**

Ghidra decompilation of the standalone UT803.exe and UT804.exe PC
software (2026-04-10) revealed that both meters use a **14-byte LCD
segment protocol** (FS9721/FS9922 family). The evidence:

1. Two `CMP EBX, 14` loops in the HID data receive callback
   (UT803: VA 0x55D0C1/0x55D0F0; UT804: VA 0x560BC1/0x560BF0)
   — frame assembly counts exactly 14 bytes.
2. Validation string `"123456789ABCDE"` (UT803: VA 0x55F091;
   UT804: VA 0x560CC0) — the 14 valid FS9721 byte index nibbles
   (high nibble 0x1–0xE).
3. 7-segment lookup table in the display handler that decodes 56
   LCD segment bits (14 nibbles × 4 bits) into digit values.
4. The UT803 manual documents RS-232 at 19200/7/Odd — the standard
   FS9721 serial format.

The UCI SDK's 0xAC/0xABCD auto-detect for QinHeng VID:PID may be
aspirational, for a different firmware version, or may require a
mode switch that the standalone apps do not perform. [KNOWN]

---

## 3. UT8802 Wire Protocol -- [VENDOR]

### 3.1 Frame Format

The UT8802 uses a simpler wire format than the UT8803:

```
+------+------+------+------+------+------+------+------+
| 0xAC | pos  | d1d2 | d3d4 | d5xx | dp+f | stat | sign |
+------+------+------+------+------+------+------+------+
 byte 0 byte 1 byte 2 byte 3 byte 4 byte 5 byte 6 byte 7
```

| Byte | Field | Extraction | Purpose |
|------|-------|------------|---------|
| 0 | Header | Must equal `0xAC` | Frame sync |
| 1 | Position | Raw byte | Rotary switch position code (0x01-0x2D) |
| 2 | Digits 1-2 | High nibble (`>>4`): digit 1; Low nibble (`&0xF`): digit 2 | BCD display digits |
| 3 | Digits 3-4 | High nibble (`>>4`): digit 3; Low nibble (`&0xF`): digit 4 | BCD display digits |
| 4 | Digit 5 | Low nibble (`&0xF`): digit 5; High nibble: unused | 5th BCD digit |
| 5 | DP + Flags | Low nibble (`&0xF`): decimal point position (0-4); Bits 4-5 (`>>4 & 3`): mode flags | Decimal placement + AC/DC |
| 6 | Status | All 8 bits extracted individually | Bargraph or secondary status [UNVERIFIED] |
| 7 | Sign + Flags | Bit 7 (`>>7`): polarity (1=negative); Bits 0-6: status flags | Sign + HOLD/REL/MAX/MIN/AUTO |

**Frame size**: Fixed 8 bytes. No length byte, no checksum. [VENDOR]

**Validation**: Only the header byte (must be 0xAC) and position code
(must be valid in FUN_1001c7b0) are checked. There is no checksum
verification. [VENDOR]

### 3.2 BCD Display Encoding -- [VENDOR]

The 5 BCD nibbles are extracted in order:
1. Byte 2 high nibble → digit 1 (most significant)
2. Byte 2 low nibble → digit 2
3. Byte 3 high nibble → digit 3
4. Byte 3 low nibble → digit 4
5. Byte 4 low nibble → digit 5 (least significant)

**BCD-to-ASCII conversion**:
- Standard digits (0-9): add 0x30 (`'0'`) to get ASCII
- Leading zeros: replaced with space (0x20)
- Nibble value 0x0A: treated as zero (`'0'`, 0x30)
- Nibble value 0x0C: converted to `'L'` (0x4C) -- overload indicator

**Decimal point insertion**: The decimal point position from byte 5 low
nibble determines where `'.'` is inserted. Position 0 = no decimal
(integer), position 1 = one digit after decimal, etc. Maximum
position is 4. The `'.'` character is inserted at
`(end_of_digits - decimal_position)`. [VENDOR]

**Overload detection**: When any digit nibble is 0x0C (`'L'`), the
overload flag (bit 7 of status word) is set and the display string
becomes `"  0L "`. [VENDOR]

**Sign**: Byte 7 bit 7 determines polarity. When set, the parsed
numeric value is negated (multiplied by -1.0). [VENDOR]

### 3.3 Position Code Table -- [KNOWN] + [VENDOR]

The UT8802 uses combined function+range position codes, unlike the
UT8803 which separates mode and range bytes.

**From the programming manual** (page 10, `DMFR.Flags` high 32-bit
D8-D15 for `disp?;` command):

| Code | Position | Category |
|------|----------|----------|
| 0x01 | DC voltage 200mV | Voltage |
| 0x03 | DC voltage 2V | Voltage |
| 0x04 | DC voltage 20V | Voltage |
| 0x05 | DC voltage 200V | Voltage |
| 0x06 | DC voltage 1000V | Voltage |
| 0x09 | AC voltage 2V | Voltage |
| 0x0A | AC voltage 20V | Voltage |
| 0x0B | AC voltage 200V | Voltage |
| 0x0C | AC voltage 750V | Voltage |
| 0x0D | DC current 200uA | Current |
| 0x0E | DC current 2mA | Current |
| 0x10 | AC current 2mA | Current |
| 0x11 | DC current 20mA | Current |
| 0x12 | DC current 200mA | Current |
| 0x13 | AC current 20mA | Current |
| 0x14 | AC current 200mA | Current |
| 0x16 | DC current 2A | Current |
| 0x18 | AC current 20A | Current |
| 0x19 | Resistance 200 ohm | Resistance |
| 0x1A | Resistance 2k ohm | Resistance |
| 0x1B | Resistance 20k ohm | Resistance |
| 0x1C | Resistance 200k ohm | Resistance |
| 0x1D | Resistance 2M ohm | Resistance |
| 0x1F | Resistance 200M ohm | Resistance |
| 0x22 | Duty ratio | Duty |
| 0x23 | Diode | Diode |
| 0x24 | Continuity | Continuity |
| 0x25 | hFE | hFE |
| 0x27 | Capacitance nF | Capacitance |
| 0x28 | Capacitance uF | Capacitance |
| 0x29 | Capacitance mF | Capacitance |
| 0x2A | SCR (thyristor) | SCR |
| 0x2B | Frequency Hz | Frequency |
| 0x2C | Frequency kHz | Frequency |
| 0x2D | Frequency MHz | Frequency |

**Gaps in the position code space** (unmapped, return 0xFF = invalid):
0x02, 0x07, 0x08, 0x0F, 0x15, 0x17, 0x1E, 0x20, 0x21, 0x26.

**Cross-reference with decompilation** (FUN_1001c7b0, line 23234):

The switch statement maps position codes to abstract functional codes:

| Position Codes | Functional Code | Function |
|----------------|-----------------|----------|
| 0x01,0x03-0x06 | 0 (Voltage) | DC Voltage ranges |
| 0x09-0x0C | 0 (Voltage) | AC Voltage ranges |
| 0x0D,0x0E,0x10-0x14,0x16,0x18 | 9 (Current) | DC/AC Current ranges |
| 0x19-0x1D,0x1F | 1 (Resistance) | Resistance ranges |
| 0x22 | 11 (Duty) | Duty ratio |
| 0x23 | 2 (Diode) | Diode test |
| 0x24 | 3 (Continuity) | Continuity test |
| 0x25 | 8 (hFE) | Transistor hFE |
| 0x27-0x29 | 4 (Capacitance) | Capacitance ranges |
| 0x2A | 12 (SCR) | Thyristor test |
| 0x2B-0x2D | 5 (Frequency) | Frequency ranges |

Note: The decompiled switch statement matches the programming manual
exactly, confirming the position code assignments. [VENDOR]

### 3.4 Byte 5 Flags (Bits 4-5) -- [VENDOR]

Byte 5 bits 4-5 (`>>4 & 3`) encode the AC/DC coupling status, used in
combination with byte 7 flags to construct the ACDC field in the status
word:

| Value | Meaning |
|-------|---------|
| 0 | OFF (no AC/DC) |
| 1 | AC or specific direction (context-dependent) |
| 2 | DC |
| 3 | AC+DC |

For diode mode (position 0x23), these bits encode probe direction:
- Value 0: both directions valid (bits 2+3 of high status set)
- Value 1: left-to-right only (bit 2 set)

For SCR mode (position 0x2A), similar direction encoding applies.
[VENDOR, partially UNVERIFIED due to Ghidra decompiler artifacts]

### 3.5 Byte 7 Status Flags -- [VENDOR]

Byte 7 carries the sign bit and multiple status flags. The exact bit
assignments are complicated by Ghidra stack variable aliasing, but the
debug format string confirms these fields are extracted:

```
"ACDC:%s,dotpos:%d,fun:%d,isauto:%d,ismax:%d,ismin:%d,ishold:%d,isrel:%d,isOL:%d,diodeDirectio:0x%x"
```

Known bit assignments (resolved 2026-04-19 from a second Ghidra pass
over `FUN_1001e0a0`, status-word construction at lines 24768-24773 and
debug format at line 24865):

- **Bit 0**: MIN → D29
- **Bit 1**: MAX → D28
- **Bit 2**: AUTO, **inverted logic** (bit clear = auto ON). Vendor
  expression `(byte)~(byte)((uint)param_2[7] >> 2) & 1` maps this to
  status-word D6.
- **Bit 3**: REL → D30
- **Bit 4**: HOLD → D31
- **Bit 5**: Over-range → D18 (not currently surfaced in `StatusFlags`)
- **Bit 6**: OL → D7
- **Bit 7**: Sign/polarity (1 = negative) → D19. Vendor multiplies the
  parsed float by -1 when set.

[VENDOR]. Real-device confirmation is still pending — the mapping is
deterministically derived from the decompile but has not been witnessed
on hardware.

### 3.6 Byte 6 Purpose -- [UNVERIFIED]

Byte 6 is passed to `FUN_1001b9b0(byte6, 0)` which constructs a bitset
from its individual bits. This function likely builds a bargraph or
secondary status indicator. The result is stored but its exact
consumption is unclear from the parser alone.

### 3.7 Status Word Construction -- [VENDOR]

The parser builds a 32-bit status word at `param_4 + 0x14` matching the
common status bit layout from the programming manual (section 3.2 of
the UT8803 protocol doc). The flag format string confirms:

| Bits | Field | Extraction |
|------|-------|------------|
| D0-D3 | FuncCode | From FUN_1001c7b0 (position → function lookup) |
| D4-D5 | ACDC | From byte 5 bits 4-5 + byte 7 bits 0-1 |
| D6 | AutoRange | From byte 7 bit 2 (**inverted**) |
| D7 | OverLoad | From digit nibble == 0x0C detection |
| D8-D11 | UnitType | From FUN_1001cf30 (position → unit lookup) |
| D12-D14 | UnitMag | From FUN_1001cd30 (position → prefix lookup) |
| D19 | Minus | From byte 7 bit 7 (sign/polarity) — see §3.5 |
| D24-D27 | ScalePos | From byte 5 low nibble (decimal position) |
| D28 | MAX | From byte 7 bit 1 — see §3.5 |
| D29 | MIN | From byte 7 bit 0 — see §3.5 |
| D30 | REL | From byte 7 bit 3 — see §3.5 |
| D31 | HOLD | From byte 7 bit 4 — see §3.5 |

The high 32-bit status word (at `param_4 + 0x18`) is also constructed,
with D2-D3 encoding diode/SCR probe direction from byte 5 bits 4-5.

---

## 4. QinHeng HID Init -- [KNOWN]

### 4.1 CH9325 Feature Report Format

Cross-referenced against the [sigrok CH9325 wiki](https://sigrok.org/wiki/WCH_CH9325),
[Lukas Schwarz's UT61B analysis](https://lukasschwarz.de/ut61b), and the
[HE2325U driver](https://github.com/thomasf/uni-trend-ut61d/blob/master/he2325u/he2325u.cpp).

The CH9325 SET_REPORT (feature report) configures UART parameters:

| Byte | Field | Notes |
|------|-------|-------|
| 0 | Report ID | Always 0x00 |
| 1-2 | Baud rate | uint16 LE |
| 3-4 | Parity/stop bits | Often 0x03/0x00; exact encoding uncertain |
| 5 | Data bits | 0=5bit, 1=6bit, 2=7bit, 3=8bit |
| 6-9 | Padding | Zeros |

Supported baud rates: 2400, 4800, 9600, 19200.

### 4.2 CH9325 HID Data Framing

**Different from CP2110 and CH9329.** All HID reports are **8 bytes**.

**RX (device→host)**: first byte = `0xF0 + payload_length`, then up to
7 payload bytes, zero-padded to 8 bytes total.
Example: `F2 35 41 00 00 00 00 00` = 2 bytes of UART data (0x35, 0x41).

**TX (host→device)**: first byte = `payload_length`, then up to 7
payload bytes, padded to 8 bytes total.

**Max 7 UART bytes per HID report** (vs 63 for CP2110/CH9329). This
means protocol frames span multiple HID reports and must be reassembled.

### 4.3 Primary Init (FUN_1001d360)

Used as the first attempt for QinHeng devices (VID 0x1A86, PID 0xE008):

```
1. Send 10-byte feature report via HidD_SetFeature:
   [0x00, 0x60, 0x09, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
   → Baud rate: 0x0960 = 2400 baud, config byte: 0x03

2. Send 0x5A trigger byte via UART write (1000ms timeout)

3. Set HID input buffer count to 64 (HidD_SetNumInputBuffers)

4. Set application buffer size to 512 bytes
```

**Feature report encoding**: The 10-byte buffer is constructed from
`local_20 = 0x3096000` (LE qword) + `local_18 = 0` (16 bits). Byte
layout: `00 60 09 03 00 00 00 00 00 00`. Bytes 1-2 = 0x0960 =
**2400 baud** (confirmed by CH9325 baud encoding: uint16 LE).

### 4.4 Fallback Init (FUN_1001d270)

Used when primary init fails to receive data within 300ms:

```
1. Send 10-byte feature report via HidD_SetFeature:
   [0x00, 0x00, 0x4B, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
   → Baud rate: 0x4B00 = 19200 baud, config byte: 0x03

2. No trigger byte sent

3. Set HID input buffer count to 64

4. Set application buffer size to 512 bytes
```

**Key differences from primary**:
- **19200 baud** instead of 2400 baud
- No 0x5A trigger byte -- the device may stream automatically at this
  baud rate

The vendor DLL probes two baud rates: 2400 first (common for older
UNI-T meters like UT61B/UT61E), then 19200 as fallback. The UCI bench
meters (UT632, UT803, UT804) respond at whichever rate they support.
This differs from the CP2110 path which hard-codes 9600 baud.

### 4.5 QinHeng Chip Identity

The chip is identified by VID 0x1A86 (QinHeng/WCH), PID 0xE008.
The specific chip model is CH9325 or its predecessor HE2325U (both
use the same USB VID/PID and compatible HID protocol). Confirmed by
cross-referencing with the [sigrok CH9325 wiki](https://sigrok.org/wiki/WCH_CH9325)
and the UT-D04 cable identification. No chip model string was found
in the decompilation.

### 4.6 Comparison: CP2110 vs QinHeng Init

| Aspect | CP2110 (UT8802/UT8803) | QinHeng (UT632/803/804) |
|--------|------------------------|------------------------|
| Feature reports | 0x41 enable + 0x50 config (AN434 format) | 10-byte: report_id + baud_16LE + config |
| Baud rate | 9600 (hard-coded) | 2400 (primary), 19200 (fallback) |
| Data framing | [length, payload...] 64 bytes | [0xF0+len, payload...] 8 bytes, max 7 UART bytes |
| Trigger | 0x5A byte after UART config | 0x5A byte (primary) or none (fallback) |
| Purge | Not sent (unlike UT61E+) | Not applicable |
| Buffer size | 3072 bytes (0xC00) | 512 bytes (0x200) |
| HID input buffers | 64 | 64 |
| Probe timeout | 300ms (UT8802) / 2000ms (UT8803) | 300ms (both primary and fallback) |

---

## 5. Serial Transport (UT805A/UT805N) -- [KNOWN] + [UNVERIFIED]

### 5.1 Configuration -- [KNOWN]

From the programming manual address string:

```
[C:DM][D:T805A][T:COM][PORT:8][BAUD:9600][PARITY:N][STOP:1][DATA:7]
```

| Parameter | Value | Notes |
|-----------|-------|-------|
| Baud rate | 9600 | Same as CP2110 models |
| Data bits | **7** | Unusual -- most serial DMMs use 8 |
| Parity | None | |
| Stop bits | 1 | |

### 5.2 Implementation Status -- [KNOWN]

The programming manual explicitly notes: "query currently only support
the device based on HID communication, COM communication does not yet
add in query system." This means the UCI SDK's `data?;` and `disp?;`
commands were not implemented for the UT805A serial path at the time
of the V1.1 manual.

### 5.3 Serial Classes in uci.dll -- [VENDOR]

The DLL contains a `CSerialPort` class (line 48010) wrapping Win32
serial APIs:
- `CreateFileW` to open COM port
- `BuildCommDCBW` with format string `"baud=%d parity=%c data=%d stop=%d"`
- `SetCommState` to apply configuration
- DTR control enabled (DTR_CONTROL_ENABLE)

The default serial configuration in the DLL is **9600/8N1** (not 7 data
bits). The 7-bit data mode from the programming manual may be
configured dynamically via the address string parameters. [VENDOR]

### 5.4 Serial Frame Format -- [UNVERIFIED]

The DLL contains three serial frame parsers (FUN_1001d960, FUN_1001db70,
FUN_1001de60) that handle FS9721-style protocols (0xF0|segment header
bytes, CR/LF delimiters). However, these parsers appear to be for
**older UNI-T models** (UT61E non-plus, UT60E, etc.) that use the
FS9721/FS9922 chipset, not the UCI bench DMMs.

The actual serial frame format used by the UT805A for UCI communication
is unknown. Possible scenarios:
1. Same 0xAC or 0xABCD binary frames as HID models, sent over serial
2. A different text-based or BCD protocol specific to serial
3. The FS9721-style protocol (less likely given the UCI SDK context)

Without a UT805A device or serial capture, this remains unverified.

### 5.5 UT805A Dual Display -- [KNOWN]

The programming manual notes that the UT805A/UT805N buffer area for
`data?;` "can set to 8Bytes or 16Bytes, that is the main display and
secondary display." When set to 16 bytes, it returns two consecutive
doubles (main + secondary display values). Other models return only
one 8-byte double. [KNOWN]

---

## 6. Per-Model Tables

### 6.1 UT8802/UT8802N Position Coding -- [KNOWN]

See section 3.3 above. The UT8802 uses combined function+range position
codes (0x01-0x2D), documented in the programming manual page 10.

### 6.2 UT804/UT804N Range Coding -- [KNOWN]

From the programming manual (page 12). Range coding is by index
(0-8), with each column representing a measurement function:

| Code | ACV | DCV | OHM | CAP | degC | uA | mA | 10A | Diode | FREQ | degF |
|------|-----|-----|-----|-----|------|----|----|-----|-------|------|------|
| 0 | 400mV | 400mV | 400 | -- | 1000 | 400u | 40mA | 10A | | 40 | 1832 |
| 1 | 4V | 4V | 4K | 40nF | | 4000u | 400mA | | | 400 | |
| 2 | 40V | 40V | 40K | 400nF | | | | | | 4K | |
| 3 | 400V | 400V | 400K | 4uF | | | | | | 40K | |
| 4 | 1000V | 1000V | 4M | 40uF | | | | | | 400K | |
| 5 | | | 40M | 400uF | | | | | | 4M | |
| 6 | | | | 4mF | | | | | | 40M | |
| 7 | | | | 40mF | | | | | | 400M | |
| 8 | | | | | | | | | | | |

Notes:
- Temperature and 10A are single-range
- Diode is fixed range (no coding needed)
- Frequency has 8 ranges (40Hz to 400MHz)
- Capacitance has 8 ranges (40nF to 40mF)

### 6.3 UT805A/UT805N Range Coding -- [KNOWN]

From the programming manual (page 12):

| Code | DCV | ACV&ACV+DCV | DCI | ACI&ACI+DCI | OHM | CAP | FREQ | Others |
|------|-----|-------------|-----|-------------|-----|-----|------|--------|
| 0 | 200mV | 200mV | 2mA | 2mA | 200 | 6nF | 6KHz | |
| 1 | 2V | 2V | 200mA | 200mA | 2K | 60nF | 60KHz | |
| 2 | 20V | 20V | 10A | 10A | 20K | 600nF | 600KHz | All range |
| 3 | 200V | 200V | | | 200K | 6uF | 6MHz | |
| 4 | 1000V | 750V | | | 2M | 60uF | 60MHz | |
| 5 | | | | | 20M | 600uF | | |
| 6 | | | | | | 6mF | | |

Notes:
- DCV has 5 ranges, ACV/ACV+DCV has 5 ranges (750V max for AC)
- DCI/ACI have 3 ranges each (2mA, 200mA, 10A)
- The "Others" column with "All range" at code 2 suggests temperature,
  diode, and continuity are single-range modes
- UT805A supports ACV+DCV and ACI+DCI combined measurements

### 6.4 UT632/UT632N and UT803/UT803N -- [KNOWN]

The programming manual lists these models in the support table with
QinHeng HID transport but does not provide model-specific position or
range coding tables. Their coding likely follows a subset of the UT804
table (since they are simpler bench meters) or uses the common
functional coding directly.

Per-model user manuals would provide measurement ranges. Without them,
the exact range tables are [UNVERIFIED].

### 6.5 Functional Coding (Common to All Models) -- [KNOWN]

All UCI bench models share the same 14 functional codes:

| Code | Function |
|------|----------|
| 0 | Voltage |
| 1 | Resistance (OHM) |
| 2 | Diode |
| 3 | Continuity |
| 4 | Capacitance |
| 5 | Frequency |
| 6 | Temperature Fahrenheit |
| 7 | Temperature Centigrade |
| 8 | Triode hFE |
| 9 | Current |
| 10 | % (4-20mA) |
| 11 | Duty ratio |
| 12 | Thyristor (SCR) |
| 13 | Inductance (incl. Q and R) |

Note: Inductance (code 13) is only available on UT8803. The UT8802 and
QinHeng models do not have inductance capability.

---

## 7. Implementation Considerations

### 7.1 Multi-Transport Support

A cross-platform implementation needs to handle three transport paths:

1. **CP2110 HID** (UT8802, UT8803): Same transport as UT61E+.
   Initialize with feature reports 0x41 + 0x50 (same as UT61E+, minus
   the purge). Send 0x5A trigger byte. Read continuously.

2. **QinHeng HID** (UT632, UT803, UT804): Different chip, different
   feature report format. Try primary init with trigger, fall back to
   no-trigger if no data received within 300ms. Auto-detect wire format
   from first frame header.

3. **Serial** (UT805A): Standard COM port at 9600 baud. Data bits may
   be 7 (per manual) or 8 (per DLL default). Wire format unknown.

### 7.2 Wire Format Detection

The CP2110 path uses a heuristic: try UT8802 parser first (shorter
timeout), then UT8803 parser. The QinHeng path auto-detects from frame
headers. An implementation should:

1. If VID:PID is 10C4:EA80: try parsing as both 0xAC and 0xABCD
2. If VID:PID is 1A86:E008: scan for header bytes in incoming data
3. Once detected, lock to that format for the session

### 7.3 UT8802 vs UT8803 Key Differences

| Aspect | UT8802 (0xAC) | UT8803 (0xABCD) |
|--------|---------------|-----------------|
| Header | 1 byte: 0xAC | 2 bytes: 0xAB 0xCD |
| Frame size | Fixed 8 bytes | Fixed 21 bytes |
| Display encoding | BCD nibbles (5 digits) | Raw bytes (5 bytes) |
| Mode/range | Combined in position code | Separate mode + range bytes |
| Checksum | None | Alternating-byte sum, 16-bit BE |
| Decimal point | Byte 5 low nibble | Implied by range |
| Sign | Byte 7 bit 7 | Flag byte bit |
| Inductance support | No | Yes (L/Q/R sub-measurements) |
| Display count | 2000 (4.5 digits) | 6000 (5999) |

### 7.4 Device Discrimination

All CP2110-based meters (UT61E+, UT61B+, UT61D+, UT161x, UT8802,
UT8803) share VID 0x10C4, PID 0xEA80. Discrimination must happen at
the application layer:

- UT61E+/B+/D+/UT161x: polled protocol (send request, get response)
- UT8802: streaming after 0x5A trigger, 0xAC frames
- UT8803: streaming after 0x5A trigger, 0xABCD frames

An implementation could:
1. Send the UT61E+ measurement request (`AB CD 03 5E 01 D9`)
2. If a valid response arrives: it's a UT61E+ family device
3. If no response: send 0x5A trigger and listen for streaming data
4. Detect 0xAC vs 0xABCD from first frame header

---

## 8. Confidence Summary

### Fully Confirmed ([KNOWN] or [VENDOR])

| Finding | Level | Source |
|---------|-------|--------|
| UT8802: 0xAC header, 8-byte fixed frame | [VENDOR] | Ghidra parser FUN_1001e0a0 |
| UT8802: BCD nibble display encoding | [VENDOR] | Ghidra digit extraction |
| UT8802: no checksum | [VENDOR] | Ghidra: no checksum code |
| UT8802: position codes 0x01-0x2D | [KNOWN] | Programming manual page 10 |
| UT8802: position-to-function mapping | [VENDOR] | Ghidra FUN_1001c7b0 matches manual |
| UT8802: AUTO flag inverted logic | [VENDOR] | Ghidra: `~(byte7>>2)` |
| QinHeng: primary init = 2400 baud + 0x5A trigger | [KNOWN] | Ghidra FUN_1001d360 + sigrok CH9325 baud encoding |
| QinHeng: fallback init = 19200 baud, no trigger | [KNOWN] | Ghidra FUN_1001d270 + sigrok CH9325 baud encoding |
| QinHeng: VID 0x1A86, PID 0xE008 | [KNOWN] | Programming manual |
| CH9325 HID data framing: 8-byte reports, 0xF0+len RX | [KNOWN] | sigrok CH9325 wiki |
| CH9325 feature report baud encoding: uint16 LE | [KNOWN] | sigrok + Lukas Schwarz UT61B + HE2325U driver |
| QinHeng wire format in UCI SDK: auto-detected 0xAC/0xABCD | [VENDOR] | Ghidra FUN_1001eb30 |
| UT803/UT804 actual wire format: FS9721 framing with proprietary structured data (NOT LCD segments) | [KNOWN] | Ghidra decompilation + binary constant extraction from UT803.exe/UT804.exe (2026-04-10) |
| UT803/UT804 init: 2400 baud via CH9325 feature report | [KNOWN] | Ghidra UT804.exe FUN_00560668 |
| UT804 range coding table | [KNOWN] | Programming manual page 12 |
| UT805A range coding table | [KNOWN] | Programming manual page 12 |
| UT805A: serial port, 9600 baud | [KNOWN] | Programming manual |
| UT805A: 7 data bits (per address string) | [KNOWN] | Programming manual |
| UT805A: dual display (2x double) | [KNOWN] | Programming manual |
| UCI serial query not implemented in SDK | [KNOWN] | Programming manual note |
| CP2110 init: same as UT61E+ minus purge | [VENDOR] | Ghidra FUN_1001d460 |
| Common functional coding (14 codes) | [KNOWN] | Programming manual |

### Requires Verification ([UNVERIFIED])

| Finding | Question |
|---------|----------|
| ~~QinHeng feature report baud rate encoding~~ | **RESOLVED**: primary=2400 baud (0x0960 LE), fallback=19200 baud (0x4B00 LE) |
| ~~Which wire format per QinHeng model~~ | **RESOLVED**: UT803/UT804 use FS9721 14-byte framing with proprietary structured data (NOT LCD segments, NOT 0xAC/0xABCD). Confirmed by binary constant extraction (2026-04-10). UCI SDK's auto-detect is for the SDK only. |
| UT805A serial frame format | UT805A manual documents ASCII text protocol (10-byte frames + CR/LF, single-letter commands). NOT the same as HID models. |
| ~~UT805A 7-bit vs 8-bit data~~ | **RESOLVED**: UT805A manual says 9600/8N1. USB is virtual COM port (not HID). |
| UT8802 byte 6 purpose | Bargraph? Secondary status? |
| ~~UT8802 byte 7 exact bit assignments~~ | **RESOLVED**: MIN=bit 0, MAX=bit 1, AUTO=bit 2 (inverted), REL=bit 3, HOLD=bit 4, Sign=bit 7. See §3.5. |
| UT8802 diode/SCR direction flags | Ghidra decompiler artifacts in comparison values |
| UT803/UT804 proprietary nibble encoding | Mode codes, range codes, digit values, sign encoding, nibbles 12-14 — all need verification against real hardware. See `docs/research/ut803/reverse-engineered-protocol.md` |
| UT805A ASCII protocol | Fully documented in manual but not yet implemented (needs serial transport) |
