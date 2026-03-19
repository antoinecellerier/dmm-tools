# UT171 Family Protocol: Reverse-Engineered Specification

Based on:
- UT171A/B/C User Manual (UNI-T)
- UT171C PC Software (UNI-T) -- Ghidra decompilation of UT171C.exe (881K lines)
- SLABHIDtoUART.dll (Silicon Labs) -- Ghidra decompilation (13K lines)
- UT171C Software Installation Instruction (UNI-T)

Cross-referenced against:
- [gulux/Uni-T-CP2110](https://github.com/gulux/Uni-T-CP2110) (Python,
  USB sniffing of UT171A)

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
| Data logging | 9,999 records | 9,999 records | 9,999 records |
| USB | CP2110 | CP2110 | CP2110 |
| Bluetooth | No | Yes (UT-D07A/B) | Yes (UT-D07A/B) |
| Conductance (nS) | No | Yes | Yes |
| Temperature | No | Yes (K-type) | Yes (K-type) |
| Square wave out | No | No | Yes |
| 600A clamp | No | No | Yes |
| Bar graph | 21 segments | 31 segments | 31 segments |

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
+------+------+------+-----------+--------+--------+
| 0xAB | 0xCD | len  | payload   | chk_lo | chk_hi |
+------+------+------+-----------+--------+--------+
 byte 0 byte 1 byte 2 bytes 3..   len+2    len+3
```

| Field | Size | Encoding | Description |
|-------|------|----------|-------------|
| Header | 2 | Fixed | `0xAB 0xCD` |
| Length | 1 | uint8 | Payload byte count |
| Payload | N | Variable | Command or response data |
| Checksum | 2 | uint16 LE | Sum of bytes[2..len+2) |

### 3.2 Checksum Algorithm -- [VENDOR]

```
checksum = sum(frame[2], frame[3], ..., frame[len+1])
frame[len+2] = checksum & 0xFF       // low byte
frame[len+3] = (checksum >> 8) & 0xFF // high byte
```

Verified against captured frames. Ghidra confirms LE byte order via
endianness-conversion functions.

### 3.3 Frame Header Validation -- [VENDOR]

From `FUN_0065478e`: Checks `*param_1 == 0xABCD`. A secondary magic
value also accepted (possibly for diagnostic frames). For non-standard
frames, sub-fields at offsets 1 and 6 must be <= 3.

### 3.4 Valid Frame Sizes -- [VENDOR]

| Length | Total | Purpose |
|--------|-------|---------|
| 0x03 | 8 | Simple command (connect/pause, query count) |
| 0x04 | 9 | Single-parameter command (delete) |
| 0x0A | 15 | Data logging read command |
| 0x11 | 22 | Standard measurement response |
| 0x12 | 23 | Start auto-save command |
| 0x17 | 28 | Extended measurement response |

---

## 4. Commands (Host → Meter) -- [VENDOR]

### 4.1 Command Frame Builder -- [VENDOR]

From `FUN_00755400`:

```
buf[0] = 0xAB
buf[1] = 0xCD
buf[2] = length
buf[3] = command_id
buf[4..] = payload
[checksum appended]
```

### 4.2 Command Categories

| Command ID | Length | Payload | Purpose |
|------------|--------|---------|---------|
| 0x01 | 0x12 (18) | 3 fields: `name;interval;flag` (7+7+1 bytes) | Start auto-save/recording |
| 0x51 | 0x0A (10) | 6 bytes: index/offset data | Read saved data by index |
| 0x52 | 0x0A (10) | 6 bytes: index/offset data | Read recording data |
| 0xFF | 0x04 (4) | 1 byte: index or 0xFF=all | Delete records |
| Others | 0x03 (3) | None | Simple commands |

### 4.3 Known Simple Commands (length = 0x03)

**Connect (start streaming):** `AB CD 04 00 0A 01 0F 00`

**Pause (stop streaming):** `AB CD 04 00 0A 00 0E 00`

Other simple commands (exact IDs [UNVERIFIED]):
- Save current measurement
- Stop auto-save
- Query saved data count
- Query recording count

### 4.4 Start Auto-Save (Command 0x01) -- [VENDOR]

```
AB CD 12 01 [7 bytes: field0] [7 bytes: field1] [1 byte: field2] [checksum]
```

Three semicolon-separated parameters from the application:
- Field 0 (bytes 4-10): date/name string (7 bytes)
- Field 1 (bytes 11-17): interval/time string (7 bytes)
- Field 2 (byte 18): integer parameter (skip-repeat flag?)

Default config from `sys.ini`: interval=3s (changed to 10 in code),
duration=60min, skipRepeat=0.

### 4.5 Read Commands (0x51, 0x52) -- [VENDOR]

```
AB CD 0A 51 [6 bytes: index/offset] [checksum]
AB CD 0A 52 [6 bytes: index/offset] [checksum]
```

The 6-byte payload likely encodes uint16 index + uint32 offset (by
analogy with UT181A). The application enforces "Query Data Count First"
before reads. [UNVERIFIED: exact byte packing]

### 4.6 Delete Command (0xFF) -- [VENDOR]

```
AB CD 04 FF [1 byte: index_or_flag] [checksum]
```

Index = specific record, 0xFF = delete all (by analogy with UT181A).

### 4.7 Mode Transition Commands -- [VENDOR]

From `FUN_00630e0b`: Switching between modes generates specific
command codes sent as the command byte:

| From | To | Command |
|------|----|---------|
| VDC (0x02) | AAC (0x18) | 0x10A |
| VDC (0x02) | NCV (0x24) | 0x110 |
| VDC (0x02) | Diode (0x0B) | 0xDA |
| Diode (0x0B) | Ohm (0x0A) | 0xD9 |
| Diode (0x0B) | VDC (0x02) | 0xDC |
| AAC (0x18) | VDC (0x02) | 0x10B |
| AAC (0x18) | Ohm (0x0A) | 0x109 |
| NCV (0x24) | VDC (0x02) | 0x111 |
| NCV (0x24) | Ohm (0x0A) | 0x10F |

Square wave output (UT171C): pseudo-mode 0x1007, commands 0xE0/0xE1.

---

## 5. Measurement Response -- [VENDOR]

### 5.1 Standard Frame (22 bytes, length = 0x11)

| Offset | Size | Field | Description | Confidence |
|--------|------|-------|-------------|------------|
| 0-1 | 2 | Header | `0xAB 0xCD` | [VENDOR] |
| 2 | 1 | Length | `0x11` (17) | [VENDOR] |
| 3 | 1 | Reserved | `0x00` | [VENDOR] |
| 4 | 1 | Type | `0x02` (measurement data) | [VENDOR] |
| 5 | 1 | Flags | Status bits (see 5.3) | [VENDOR] |
| 6 | 1 | Frame type | 0x01=standard, 0x03=extended | [VENDOR] |
| 7 | 1 | Mode | Measurement type (see 6) | [VENDOR] |
| 8 | 1 | Range | Range index (raw, 1-based) | [VENDOR] |
| 9-12 | 4 | Main value | IEEE 754 float32, LE | [VENDOR] |
| 13 | 1 | Status2 | 0x40=DC, 0x20=AC | [DEDUCED] |
| 14 | 1 | Unknown | Values 0x00, 0x01 | [UNVERIFIED] |
| 15-18 | 4 | Aux value | IEEE 754 float32, LE | [VENDOR] |
| 19 | 1 | (padding) | | |
| 20-21 | 2 | Checksum | uint16 LE | [VENDOR] |

### 5.2 Extended Frame (28 bytes, length = 0x17)

All standard fields plus:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 19-20 | 2 | Extra flags | Additional metadata |
| 21-24 | 4 | Third value | IEEE 754 float32, LE (AC+DC combined?) |
| 25 | 1 | Unknown | |
| 26-27 | 2 | Checksum | uint16 LE |

Byte 6 = 0x03 signals extended frame. The third float is close to the
main value in AC modes, suggesting AC+DC combined measurement. [DEDUCED]

### 5.3 Flags Byte (Offset 5) -- [VENDOR]

| Bit | Mask | Meaning | Evidence |
|-----|------|---------|----------|
| 7 | 0x80 | **HOLD** active | Ghidra: triggers recording state change |
| 6 | 0x40 | **AUTO** range (**inverted**: clear = AUTO active) | Ghidra: `(flags & 0x40) == 0` → set AUTO indicator |
| 3 | 0x08 | **Conditional flag** (MIN/MAX?), gated by bit 0 | Ghidra: sets output bit 2 when bit 0 also set |
| 2 | 0x04 | **Low battery** | Confirmed from USB captures |
| 1 | 0x02 | **Extended frame** indicator | Ghidra: selects extended vs standard frame size |
| 0 | 0x01 | **Gate** for bit 3 effect | Ghidra: secondary condition |

Bits 4-5 (0x10, 0x20): not observed in decompilation. [UNVERIFIED]

### 5.4 Range Byte (Offset 8) -- [VENDOR]

The range byte is used **raw** (no 0x0F masking, unlike UT61E+). Value
maps directly to the chart range index (1-based). Value 0 = auto.

Per-mode range tables from string extraction:

| Mode | Ranges (index: bounds) |
|------|----------------------|
| ADC | 1: ±6A, 2: ±20A |
| BEEP | 1: 0-600Ω |
| nS | 1: 0-60nS |
| Hz | 1: 0-60, 2: 0-600 |
| kHz | 3: 0-6, 4: 0-60, 5: 0-600 |
| MHz | 6: 0-6, 7: 0-60 |
| nF | 1: 0-6, 2: 0-60, 3: 0-600 |
| uF | 4: 0-6, 5: 0-60, 6: 0-600 |
| mF | 7: 0-6, 8: 0-60 |
| mADC | 1: ±60, 2: ±600 |
| uADC | 1: ±600, 2: ±6000 |
| mAAC | 1: 0-60, 2: 0-600 |
| uAAC | 1: 0-600, 2: 0-6000 |

Frequency uses continuous indexing across Hz/kHz/MHz (1-7).
Capacitance uses continuous indexing across nF/uF/mF (1-8).

---

## 6. Mode Byte Table -- [VENDOR]

Complete table from Ghidra analysis of FUN_0064081c (mode→data-log
encoding), FUN_006405b1 (data-log→mode decoding), FUN_00630c1e
(mode→storage-size), and FUN_00630d5c/dd4 (mode grouping):

| Byte | Mode | Group | Evidence |
|------|------|-------|----------|
| 0x01 | LoZ V~ (low impedance ACV) | — | FUN_006405b1: group 9→mode 1. String "LoZV" |
| 0x02 | V DC | R/V/Diode | USB confirmed. FUN_00630d3a voltage group |
| 0x03 | V AC | — | FUN_0064081c: group 0x4B+flag. String "VAC" |
| 0x04 | V AC+DC | — | FUN_0064081c: group 3. String "VAC+DC" |
| 0x05 | mV DC | AC group | USB confirmed. FUN_00630d3a voltage group |
| 0x06 | mV AC | AC group | USB confirmed. String "mVAC" |
| 0x07 | mV AC+DC | AC group | FUN_0064081c: group 4. String "mVAC+DC" |
| 0x08 | Continuity (BEEP) | AC group | FUN_0064081c: group 5. String "BEEP Range 1: 0 to 600" |
| 0x09 | Capacitance | — | FUN_006405b1: group 0x0B→mode 9. nF/uF/mF range strings |
| 0x0A | Resistance (Ω) | R/V/Diode | USB confirmed. String "OHM" |
| 0x0B | Diode | R/V/Diode | USB confirmed. String "DIOD" |
| 0x0C | Temperature °C | AC group | FUN_0064081c: group 2, range 2. String "Temp-C" |
| 0x0D | Temperature °F | AC group | FUN_0064081c: group 2, range 4. String "Temp-F" |
| 0x0E | Conductance (nS) | — | FUN_0064081c: group 4, range 10. String "nS Range 1: 0 to 60" |
| 0x0F | Frequency (Hz) | — | USB confirmed. Hz/kHz/MHz range strings |
| 0x10 | Duty cycle (%) | — | String "Duty". May share 0x0F with sub-field | [DEDUCED] |
| 0x11 | µA DC | — | USB confirmed. String "uADC Range 1" |
| 0x12 | µA AC | AC group | USB confirmed. String "uAAC Range 1" |
| 0x13 | µA AC+DC | AC group | FUN_0064081c: group 2, range 8. String "uAAC+DC" |
| 0x14 | mA DC | — | USB confirmed. String "mADC Range 1" |
| 0x15 | mA AC | — | USB confirmed. String "mAAC Range 1" |
| 0x16 | mA AC+DC | — | FUN_0064081c: group 0x0F. String "mAAC+DC" |
| 0x17 | A DC | — | USB confirmed. String "ADC Range 1" |
| 0x18 | A AC | R/V/Diode | USB confirmed. String "AAC" |
| 0x19 | A AC+DC | AC group | FUN_0064081c: group 0x12. String "AAC+DC" |
| 0x1A | VFC (V→freq converter) | — | String "VFC". Manual: long-press in AC V mode | [DEDUCED] |
| 0x1B | % (4-20mA) | AC group | FUN_0064081c: group 0x1B. String "Range 1: 4 to 20" |
| 0x1C | 600A DC (clamp) | AC group | FUN_0064081c: group 1. String "ADC600A". UT171C only |
| 0x1D | 600A AC (clamp) | AC group | FUN_0064081c: group 2. String "AAC600A". UT171C only |
| 0x24 | NCV (non-contact voltage) | R/V/Diode | FUN_0064081c: group 0x24. String "NCV". Transitions to/from VDC, OHM |
| 0x1007 | Square wave output | — | 16-bit pseudo-mode. Commands 0xE0/0xE1. UT171C only |

**Mode groups** (from decompilation):
- **AC/complex group**: 0x05, 0x06, 0x07, 0x08, 0x0C, 0x0D, 0x12,
  0x13, 0x19, 0x1B, 0x1C, 0x1D
- **R/V/Diode group**: 0x02, 0x0A, 0x0B, 0x18, 0x24
- **Voltage group**: 0x02, 0x03, 0x04, 0x05, 0x06

---

## 7. Data Logging Protocol -- [VENDOR] (partial)

### 7.1 Command Structure

| Command | Length | Format | Purpose |
|---------|--------|--------|---------|
| 0x01 | 0x12 | `[name(7);interval(7);flag(1)]` | Start auto-save |
| 0x51 | 0x0A | `[index/offset(6)]` | Read saved measurement |
| 0x52 | 0x0A | `[index/offset(6)]` | Read recording data |
| 0xFF | 0x04 | `[index(1)]` | Delete (0xFF=all) |
| Simple | 0x03 | (none) | Save current, stop, query count |

### 7.2 Operational Sequence

1. Connect to meter
2. **Query Data Count** (simple command) — required first
3. **Read records** by index (0x51/0x52) — iterate 1 to count
4. Or: **Start Auto Save** (0x01) with interval/duration
5. **Delete records** (0xFF) when done

### 7.3 Configuration Defaults

From `sys.ini`: interval=3s, duration=60min, skipRepeat=0.

---

## 8. SLABHIDtoUART.dll vs Raw HID Access -- [VENDOR]

Comparison with our existing CP2110 implementation:

| Aspect | SLAB Library | Our cp2110.rs |
|--------|-------------|---------------|
| Open sequence | Open → GetPartNumber → EnableUART | EnableUART → SetConfig → PurgeRX |
| Part number check | Yes (must be 0x0A) | No (relies on VID/PID) |
| UART enable in Open | Yes (automatic) | Explicit in init_uart() |
| SetUartConfig in Open | No (caller must do) | Done in init_uart() |
| Purge in Open | No (caller must do) | Done in init_uart() |
| Write chunking | Auto (63-byte reports, max 4096/call) | Single reports |
| Read buffering | Internal ring buffer | Single report reads |
| UART status byte order | **Big-endian** FIFO counts | Uses from_le_bytes — **potential bug** |

**Potential bug**: Our `cp2110.rs` UART status parsing at line 122-123
uses `u16::from_le_bytes` for TX/RX FIFO counts, but the SLAB DLL
decompilation shows `CONCAT11(byte[1], byte[2])` which is big-endian.
Needs device verification.

---

## 9. Key Differences from Other UNI-T Protocols

| Aspect | UT61E+ | UT171 | UT181A | UT8803 |
|--------|--------|-------|--------|--------|
| Header | AB CD | AB CD | AB CD | AB CD |
| Length | 1 byte (payload+2) | 1 byte (payload) | 2 bytes LE (payload+2) | 1 byte |
| Comm model | Polled | Streaming | Streaming | Streaming |
| Values | 7x ASCII | LE float32 | LE float32 | 5 raw bytes |
| Mode encoding | 1 byte (0x00-0x19) | 1 byte (0x01-0x24) | 2 bytes LE (0x1111-0xA231) | UCI functional code |
| Checksum | 16-bit BE | 16-bit LE | 16-bit LE | Alternating-byte |
| Range byte | 0x30 prefix, mask & 0x0F | Raw (1-based index) | Raw (0x00-0x08) | 0x30 prefix |
| AUTO flag | Inverted (clear=ON) | Inverted (clear=ON) | misc2 bit 0 | Inverted |
| Data logging | None | 9,999 records | 20,000 records | None |
| USB library | Raw HID | SLABHIDtoUART.dll | cp211x_uart crate | uci.dll |

---

## 10. Confidence Summary

### Confirmed ([KNOWN] or [VENDOR])

| Finding | Level |
|---------|-------|
| Frame header 0xABCD, 1-byte length, 16-bit LE checksum | [VENDOR] |
| 26 mode bytes mapped (24 single-byte + 2 special) | [VENDOR] |
| 13 mode bytes USB-capture confirmed | [VENDOR] |
| AUTO flag: bit 6 inverted (clear = active) | [VENDOR] |
| HOLD flag: bit 7 | [VENDOR] |
| Low battery: bit 2 | [VENDOR] |
| Frame type byte (offset 6): 0x01=standard, 0x03=extended | [VENDOR] |
| Range byte: raw 1-based index (no masking) | [VENDOR] |
| Command builder: 4 length categories | [VENDOR] |
| Data logging commands: 0x01, 0x51, 0x52, 0xFF | [VENDOR] |
| Mode transition command table | [VENDOR] |
| Complete CP2110 feature report map (20 reports) | [VENDOR] |
| UART config report 0x50 byte layout | [VENDOR] |
| 9600/8N1, VID 0x10C4, PID 0xEA80 | [VENDOR] |
| All measurement ranges and accuracy | [KNOWN] |

### Remaining Gaps ([UNVERIFIED])

| Gap | Impact |
|-----|--------|
| Mode 0x10 (Duty%): may share 0x0F with sub-field | Minor |
| Mode 0x1A (VFC): deduced from gap, not seen in data-log encoder | Minor |
| Exact simple command IDs (save, stop, query count) | Need USB capture |
| 0x51 vs 0x52 exact semantics | Need USB capture |
| Status2 byte (offset 13) meaning | Display hint only |
| Flag bits 4-5 (0x10, 0x20) | Not observed |
| UART status FIFO count endianness (cp2110.rs potential bug) | Need device test |

### Cross-Reference with Community Sources

| Finding | Our RE | gulux | Agreement |
|---------|--------|-------|:---------:|
| Frame header AB CD | Ghidra | Same | ✓ |
| 16-bit LE checksum | Ghidra + manual verify | Same | ✓ |
| LE float32 values | Ghidra | Same | ✓ |
| 13 USB-confirmed mode bytes | Ghidra mode table | Same | ✓ |
| Connect/Pause commands | Ghidra command builder | Same | ✓ |
| VID/PID, 9600 baud | Ghidra | Same | ✓ |
| Streaming model | Ghidra reader thread | Same | ✓ |
| 13 additional mode bytes | Ghidra (new) | Not in gulux | New |
| AUTO flag inverted | Ghidra (new) | Not in gulux | New |
| Data logging commands | Ghidra (new) | Not in gulux | New |
| CP2110 feature report map | SLAB DLL Ghidra (new) | Not in gulux | New |
| Mode transition commands | Ghidra (new) | Not in gulux | New |
