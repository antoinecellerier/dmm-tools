# Reverse Engineering Approach: UT171 Family Protocol

## Objective

Reconstruct the UT171A/B/C USB communication protocol using official,
publicly available sources:

1. **UT171A/B/C User Manual** (from UNI-T, `uni-t_ut171_manual.pdf`)
2. **UT171C PC Software** (from UNI-T, `UT171C setup.exe`)
3. **UT171C Software Installation Instruction** (from UNI-T)
4. **SLABHIDtoUART.dll** (from Silicon Labs, bundled with software)
5. **SLABHIDDevice.dll** (from Silicon Labs, bundled with software)

Community implementations are used only for cross-referencing after
the primary analysis is complete.

## What Each Source Provides

### UT171A/B/C User Manual

The manual defines all **application-layer semantics**:

- **Display counts**: UT171A: 40,000; UT171B/C: 60,000
- **Measurement modes**: DCV, ACV, DCA, ACA, AC+DC voltage/current,
  resistance, conductance (nS, UT171B/C only), capacitance, frequency,
  duty cycle, temperature (C/F), diode, continuity, LoZ ACV, VFC,
  % (4-20mA), NCV, square wave output (UT171C only), 600A clamp
  (UT171C only)
- **Ranges**: Complete per-mode range tables with accuracy specs
  (pages 28-34)
- **Bar graph**: 21 pointers (UT171A), 31 pointers (UT171B/C)
- **Data logging**: Up to 9,999 records
- **Update rate**: 4-5 times per second
- **Settings**: USB communication on/off, backlight, buzzer, auto power
  off, RTC date/time (UT171C)

### UT171C PC Software

The vendor software installer (`UT171C setup.exe`, InstallShield)
installs to `C:\Program Files\DMM\UT171C\` with these files:

| File | Size | Purpose |
|------|------|---------|
| UT171C.exe | 10.3 MB | Main application (Delphi, DevExpress GUI) |
| SLABHIDtoUART.dll | 80 KB | Silicon Labs CP2110 HID-to-UART library |
| SLABHIDDevice.dll | 108 KB | Silicon Labs HID device library |
| gdi32.dll | 305 KB | Custom GDI shim |
| sys.ini | 62 B | Default settings (interval, duration) |

### UT171C Software Installation Instruction

This PDF (4 pages) shows the software UI including:
- **Complete function button grid**: All measurement modes the software
  can command, organized by category
- **Toolbar**: Conn, DisConn, Start/Pause Receive Data, Hold, Hz%,
  Peak, Max Min, Parameter
- **Data management**: Save Current Data, Auto Save, Read Data by Index,
  Read All Data, Query Data Count, Delete Data by Index, Delete All Data

## Analysis Techniques Used

### 1. Download and catalog

Downloaded from `meters.uni-trend.com`:
- User manual PDF (11.2 MB)
- UT171B software ZIP (5.8 MB, contains UT171C setup.exe)

### 2. Installer extraction

The InstallShield setup.exe could not be extracted with standard tools
(7z, unshield). Wine was used to run the installer and extract the
application files:

```sh
WINEPREFIX=~/wine-ut171 wine "UT171C setup.exe" /v"/qn"
# Files installed to: drive_c/Program Files (x86)/DMM/UT171C/
```

### 3. String extraction

```sh
strings -a -el UT171C.exe | grep -iE 'mode|range|HidUart|connect'
```

Key findings:
- `HidUart_Open`, `HidUart_Read`, `HidUart_Write`, `HidUart_SetUartConfig`,
  `HidUart_Close` -- confirms Silicon Labs HidUart API usage
- Mode name strings: LoZV, VFC, VAC, VDC, mVDC, mVAC, OHM, BEEP, CAP,
  DIOD, nS, ADC, AAC, mADC, mAAC, uADC, uAAC, AAC+DC, mAAC+DC,
  uAAC+DC, mVAC+DC, VAC+DC, PLUSE_O, 600A
- Per-mode chart range strings: `ADC Range 1: -6 to 6`,
  `BEEP Range 1: 0 to 600`, `nS Range 1: 0 to 60`, etc.
- `"First Query Data Count,Please!"` -- data logging sequence
- VID/PID strings: `L"10C4"`, `L"EA80"`

### 4. Ghidra headless decompilation

```sh
~/stuff/ghidra/ghidra_12.0.4_PUBLIC/support/analyzeHeadless \
    /tmp/ghidra_ut171c2 ut171c_proj \
    -import UT171C.exe \
    -postScript GhidraDecompile.java \
    -deleteProject -scriptPath /tmp \
    > ut171c_decompiled.txt 2>&1
```

The decompilation produced 881,179 lines (28,745 functions). The
application is a Delphi/DevExpress GUI with protocol code embedded
in compiled virtual method tables.

Key functions identified:
- `FUN_0065478e` (line 297968) -- frame header validator (`!= 0xabcd`)
- `FUN_00755400` (line 397440) -- command frame builder (AB CD
  framing, length selection, payload encoding)
- `FUN_0075425c` (line 396673) -- USB HID initialization (LoadLibrary
  SLABHIDtoUART.dll, VID/PID matching, UART configuration)
- `FUN_00755228` (line 397330) -- reader thread (polling HidUart_Read,
  SendMessage to main thread with received data)
- `FUN_00630e0b` (line 267638) -- mode transition command table
  (maps mode pairs to command codes)
- `FUN_00630d5c` (line 267600) -- mode byte grouping for AC/complex
  measurement modes
- `FUN_00630dd4` (line 267624) -- mode byte grouping for
  resistance/voltage/diode modes

## What Was Determined

### From the User Manual (highest confidence)

| Finding | Confidence | Source |
|---------|------------|--------|
| UT171A: 40,000 counts; UT171B/C: 60,000 counts | **[KNOWN]** | Manual page 3 |
| All measurement ranges and accuracy specs | **[KNOWN]** | Manual pages 28-34 |
| Data logging: 9,999 records | **[KNOWN]** | Manual page 4 |
| K-type thermocouple (UT171B/C only) | **[KNOWN]** | Manual page 20 |
| Square wave output (UT171C only) | **[KNOWN]** | Manual page 25 |
| 600A current clamp (UT171C only) | **[KNOWN]** | Manual page 23 |
| Update rate: 4-5 times/second | **[KNOWN]** | Manual page 4 |
| CAT III 1000V / CAT IV 600V | **[KNOWN]** | Manual page 2 |

### From Vendor Software Analysis

| Finding | Confidence | Source |
|---------|------------|--------|
| Uses SLABHIDtoUART.dll (HidUart API) | **[VENDOR]** | String extraction + decompilation |
| VID 0x10C4, PID 0xEA80 (CP2110) | **[VENDOR]** | Decompilation: L"10C4", L"EA80" |
| Baud rate index 6 = 9600 | **[VENDOR]** | Decompilation: FUN_00754560 |
| Frame header: 0xABCD | **[VENDOR]** | Decompilation: FUN_0065478e |
| Command frame builder | **[VENDOR]** | Decompilation: FUN_00755400 |
| Command lengths: 0x03, 0x04, 0x0A, 0x12 | **[VENDOR]** | Decompilation: command builder switch |
| Data logging command 0x01 (multi-field) | **[VENDOR]** | Decompilation: semicolon-separated fields |
| Data logging commands 0x51, 0x52 | **[VENDOR]** | Decompilation: length 0x0A branch |
| Mode transition command table | **[VENDOR]** | Decompilation: FUN_00630e0b |
| Mode byte groups (AC: 0x05-0x1d; R/V: 0x02-0x24) | **[VENDOR]** | Decompilation: FUN_00630d5c/dd4 |
| 26 mode bytes mapped (via encoding/decoding/size functions) | **[VENDOR]** | Decompilation: FUN_0064081c, FUN_006405b1, FUN_00630c1e |
| NCV = mode byte 0x24 (transitions to/from VDC, OHM) | **[VENDOR]** | Decompilation: mode transition table + string "NCV" |
| AUTO flag: bit 6 inverted (clear = active) | **[VENDOR]** | Decompilation: FUN_00980070 |
| HOLD flag: bit 7; LowBat: bit 2 | **[VENDOR]** | Decompilation: FUN_0065492d + USB captures |
| Frame type byte (offset 6): 0x01=standard, 0x03=extended | **[VENDOR]** | Decompilation: FUN_00654bf5 |
| Range byte: raw 1-based index (no masking) | **[VENDOR]** | Decompilation + chart range strings |
| Reader thread: 70ms polling, SendMessage dispatch | **[VENDOR]** | Decompilation: FUN_00755228 |
| UART: 500ms timeout, 100ms read interval | **[VENDOR]** | Decompilation: FUN_00754560 |
| Complete CP2110 feature report map (20 reports) | **[VENDOR]** | Ghidra: SLABHIDtoUART.dll decompilation |
| UART config report 0x50 byte layout confirmed | **[VENDOR]** | Ghidra: SLABHIDtoUART.dll |
| UART status FIFO counts are big-endian | **[VENDOR]** | Ghidra: SLABHIDtoUART.dll report 0x42 |
| Complete function list (28 modes) | **[VENDOR]** | Software manual UI screenshots |

### Resolved by Deep Decompilation Analysis (Phase 2)

The initial decompilation pass identified key functions. A second pass
with targeted agents resolved most gaps:

| Previously unverified | Resolution | Source |
|-----------------------|------------|--------|
| Mode byte assignments (13 of 28+) | **26 modes now mapped** via FUN_0064081c (mode→encoding), FUN_006405b1 (encoding→mode), FUN_00630c1e (mode→storage size) | [VENDOR] |
| Flag byte bit positions | **AUTO (bit 6, inverted), HOLD (bit 7), LowBat (bit 2), extended-frame (bit 1), conditional MIN/MAX (bit 3 gated by bit 0)** from FUN_00980070 and FUN_0065492d | [VENDOR] |
| Range byte decoding | **Raw 1-based index** (no masking), per-mode range tables from chart strings | [VENDOR] |
| Data logging commands | **0x01=start-save (3 fields), 0x51/0x52=read ops, 0xFF=delete** with byte-level frame layouts | [VENDOR] |
| SLABHIDtoUART.dll internals | **Complete CP2110 feature report map (20 IDs)**, UART config byte layout, open sequence | [VENDOR] |

### What Still Requires Device Verification

1. **Mode 0x10 (Duty%)**: May share mode byte 0x0F with frequency,
   distinguished by a sub-field rather than a separate mode byte.
   Not present in the data-log encoding function. [UNVERIFIED]

2. **Mode 0x1A (VFC)**: Deduced from the gap in the mode byte
   sequence and the "VFC" string in the binary, but not found in
   the data-log encoding or mode group functions. [DEDUCED]

3. **Exact simple command IDs**: Save current, stop auto-save, and
   query count use the simple (length=0x03) format but their specific
   command byte values are passed via virtual dispatch and not directly
   visible in the decompilation. [UNVERIFIED]

4. **0x51 vs 0x52 exact semantics**: Both use length 0x0A with 6-byte
   payloads. Which is read-saved-measurement vs read-recording-data
   is deduced by analogy with UT181A, not confirmed. [UNVERIFIED]

5. **Flag bits 4-5 (0x10, 0x20)**: Not observed in any decompiled
   function. May be unused or accessed through virtual dispatch.
   [UNVERIFIED]

6. **UART status FIFO count endianness**: The SLABHIDtoUART.dll
   decompilation shows big-endian FIFO counts in report 0x42, but
   our cp2110.rs uses little-endian. Needs device verification.
   [UNVERIFIED]

## Cross-Reference with Community Sources

### gulux/Uni-T-CP2110 (Python, USB sniffing)

| Finding | Our RE | gulux | Agreement |
|---------|--------|-------|:---------:|
| Frame header AB CD | Ghidra: `!= 0xabcd` | Same | ✓ |
| 16-bit LE checksum | Ghidra: endian conversion functions | Verified against captures | ✓ |
| LE float32 values | Ghidra: float references in parsers | Same | ✓ |
| Mode byte at offset 7 | Ghidra: mode transition table | 13 modes confirmed | ✓ |
| Connect cmd: AB CD 04 00 0A 01 | Ghidra: command builder, length 0x03 for default | Same | ✓ |
| VID 0x10C4, PID 0xEA80 | Ghidra: string extraction | Same | ✓ |
| 9600 baud | Ghidra: baud index 6 | Same | ✓ |
| Streaming communication model | Ghidra: reader thread polls continuously | Same | ✓ |
| Data logging commands | Ghidra: 0x01, 0x51, 0x52, 0xFF | Not in gulux | New |
| 13 additional mode bytes | Ghidra: FUN_0064081c/006405b1 | Not in gulux | New |
| AUTO flag (bit 6, inverted) | Ghidra: FUN_00980070 | Not in gulux | New |
| Frame type byte (offset 6) | Ghidra: FUN_00654bf5 | Not in gulux | New |
| Range byte: raw 1-based | Ghidra + chart strings | Not in gulux | New |
| Command length variants | Ghidra: 4 different lengths | Not in gulux | New |
| CP2110 feature report map | Ghidra: SLABHIDtoUART.dll | Not in gulux | New |

## File Inventory

| Source | File | What it provides |
|--------|------|-----------------|
| UNI-T | `references/ut171/uni-t_ut171_manual.pdf` | All modes, ranges, accuracy, features |
| UNI-T | `references/ut171/vendor-software/UT171C.exe` | Main application binary (10.3 MB) |
| UNI-T | `references/ut171/vendor-software/SLABHIDtoUART.dll` | CP2110 UART library |
| UNI-T | `references/ut171/vendor-software/SLABHIDDevice.dll` | CP2110 HID library |
| UNI-T | `references/ut171/vendor-software/sys.ini` | Default config |
| Analysis | `references/ut171/vendor-software/ut171c_decompiled.txt` | Ghidra decompilation (881K lines) |
| UNI-T | `references/ut171/vendor-software/UT171B_Software.zip` | Original download |
